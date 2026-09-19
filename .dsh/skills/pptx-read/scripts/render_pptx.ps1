# pptx-read / render_pptx.ps1
#
# Render a .pptx with the REAL PowerPoint engine (COM) into per-slide PNG,
# export a PDF, and dump every slide's text to UTF-8 text.md.
#
# ISOLATION GUARANTEE -- read this before touching the COM block.
#
#   `PowerPoint.Application` is a SINGLE-INSTANCE COM server. `New-Object
#   -ComObject PowerPoint.Application` SILENTLY ATTACHES to the user's already
#   running PowerPoint when there is one. Quitting that instance closes whatever
#   deck the user had open; and leaving a window-less instance behind makes the
#   user's next double-click land in an invisible window, which reads as "I can
#   no longer open my ppt". So this script must:
#
#     (a) copy the deck fresh on every run and open ONLY the copy,
#     (b) give the copy a per-source unique name, so two same-named decks in
#         different directories can never collide inside PowerPoint,
#     (c) quit ONLY an instance it started itself (checked before COM is
#         created), and
#     (d) start each run from an empty scratch root, clearing the previous
#         round's copy.
#
# This file is deliberately ASCII-only. Windows PowerShell 5.1 reads a .ps1
# without a BOM as ANSI, which would corrupt any non-ASCII literal in it; the
# source path therefore arrives base64-encoded (UTF-8) instead of as a literal.
#
# Parameters may also be supplied through the environment so the script can be
# launched with -EncodedCommand when -File is unavailable.

param(
  [string]$SrcB64 = $env:PPTX_SRC_B64,
  [string]$Slug   = $env:PPTX_SLUG,
  [int]$Width     = $(if ($env:PPTX_WIDTH)  { [int]$env:PPTX_WIDTH }  else { 1600 }),
  [int]$Height    = $(if ($env:PPTX_HEIGHT) { [int]$env:PPTX_HEIGHT } else { 900 }),
  [int]$Render    = $(if ($env:PPTX_RENDER) { [int]$env:PPTX_RENDER } else { 1 })
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function ConvertTo-WslPath([string]$p) {
  # C:\Users\x\AppData\Local\Temp  ->  /mnt/c/Users/x/AppData/Local/Temp
  if ($p -match '^([A-Za-z]):\\(.*)$') {
    return '/mnt/' + $matches[1].ToLower() + '/' + ($matches[2] -replace '\\', '/')
  }
  return $p
}

if ([string]::IsNullOrWhiteSpace($SrcB64)) { Write-Output 'STATUS=ERR_NO_SOURCE'; exit 2 }
$src = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($SrcB64))

if (-not (Test-Path -LiteralPath $src)) { Write-Output 'STATUS=ERR_NO_SOURCE'; exit 2 }
if ([string]::IsNullOrWhiteSpace($Slug)) { $Slug = 'deck' }

# --- round lifecycle: start from an empty scratch root -----------------------
# The previous round's copy and renders are removed here, so nothing from an
# earlier deck can be mistaken for this one's output, and no stale file handle
# survives into this run.
$root = Join-Path $env:TEMP 'pptx-read'
if (Test-Path -LiteralPath $root) {
  Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Force -Path $root | Out-Null
$workDir = Join-Path $root $Slug
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

# The copy is named after the source-path slug, not after the original file:
# two decks that merely share a file name must not share an identity inside
# PowerPoint. The name stays pure ASCII so no code page can touch it.
$copyName = 'deck-' + $Slug + '.pptx'
$local    = Join-Path $workDir $copyName
Copy-Item -LiteralPath $src -Destination $local -Force
Write-Output 'STATUS=COPIED'
Write-Output ('COPY=' + $local)
Write-Output ('COPY_WSL=' + (ConvertTo-WslPath $local))

# --- COM, with instance ownership tracked BEFORE the object is created --------
$preExisting = @(Get-Process -Name POWERPNT -ErrorAction SilentlyContinue).Count -gt 0
Write-Output ('POWERPOINT_WAS_RUNNING=' + $(if ($preExisting) { 1 } else { 0 }))

$app  = $null
$pres = $null
$origAlerts = $null
try {
  $app = New-Object -ComObject PowerPoint.Application
  # When we borrowed the user's instance, leave its settings as we found them.
  try { $origAlerts = $app.DisplayAlerts; $app.DisplayAlerts = 1 } catch {}

  # Open(FileName, ReadOnly=msoTrue, Untitled=msoFalse, WithWindow=msoFalse)
  $pres = $app.Presentations.Open($local, -1, 0, 0)

  $n = $pres.Slides.Count
  $w = [math]::Round($pres.PageSetup.SlideWidth, 1)
  $h = [math]::Round($pres.PageSetup.SlideHeight, 1)
  Write-Output ("SLIDES=" + $n)
  Write-Output ("PAGESIZE_PT=" + $w + "x" + $h)

  $sb = New-Object System.Text.StringBuilder
  [void]$sb.AppendLine('# ' + [System.IO.Path]::GetFileName($src))
  [void]$sb.AppendLine()
  [void]$sb.AppendLine('- slides: ' + $n)
  [void]$sb.AppendLine('- page size (pt): ' + $w + ' x ' + $h)
  [void]$sb.AppendLine()

  foreach ($s in $pres.Slides) {
    [void]$sb.AppendLine('## Slide ' + $s.SlideIndex)
    [void]$sb.AppendLine()
    $found = 0
    foreach ($sh in $s.Shapes) {
      try {
        if ($sh.HasTextFrame -eq -1 -and $sh.TextFrame.HasText -eq -1) {
          $t = $sh.TextFrame.TextRange.Text
          $t = ($t -replace "`r`n", "`n") -replace "`r", "`n"
          $t = $t.Trim()
          if ($t.Length -gt 0) {
            $found++
            [void]$sb.AppendLine('### ' + $sh.Name)
            [void]$sb.AppendLine($t)
            [void]$sb.AppendLine()
          }
        }
      } catch {}

      # A table's text does not live in the slide-level TextFrame, so reading
      # only HasTextFrame silently drops every table on the slide -- exactly the
      # content a reader most wants. Walk the cells instead.
      try {
        if ($sh.HasTable -eq -1) {
          $tbl = $sh.Table
          $rows = $tbl.Rows.Count
          $cols = $tbl.Columns.Count
          $found++
          [void]$sb.AppendLine('### ' + $sh.Name + ' [TABLE ' + $rows + 'x' + $cols + ']')
          for ($r = 1; $r -le $rows; $r++) {
            $cells = @()
            for ($c = 1; $c -le $cols; $c++) {
              $cell = ''
              try {
                $cell = $tbl.Cell($r, $c).Shape.TextFrame.TextRange.Text
                $cell = (($cell -replace "`r`n", ' ') -replace "`r", ' ' -replace "`n", ' ').Trim()
                # An unescaped pipe would forge a column boundary.
                $cell = $cell -replace '\|', '\|'
              } catch {}
              $cells += $cell
            }
            [void]$sb.AppendLine('| ' + ($cells -join ' | ') + ' |')
          }
          [void]$sb.AppendLine()
        }
      } catch {}
    }
    if ($found -eq 0) { [void]$sb.AppendLine('(no text shapes)'); [void]$sb.AppendLine() }
  }

  $textPath = Join-Path $workDir 'text.md'
  # AppendLine emits Environment.NewLine, which is CRLF on Windows, while the
  # embedded text carries bare LF. The mixed result made `awk '/^## Slide 4$/'`
  # miss because of a trailing CR. Normalize the whole document to LF so the
  # section recipes in SKILL.md work verbatim.
  $outText = $sb.ToString() -replace "`r`n", "`n"
  [System.IO.File]::WriteAllText($textPath, $outText, (New-Object System.Text.UTF8Encoding($false)))
  Write-Output ('TEXT_MD=' + $textPath)

  if ($Render -eq 1) {
    $pres.Export($workDir, 'PNG', $Width, $Height)
    $png = Get-ChildItem -Recurse $workDir -Filter '*.PNG' -ErrorAction SilentlyContinue |
      Select-Object -First 1
    if ($png) { Write-Output ('PNG_DIR=' + $png.DirectoryName) } else { Write-Output 'PNG_DIR=' }
    $pdf = Join-Path $workDir 'deck.pdf'
    $pres.SaveAs($pdf, 32)
    Write-Output ('PDF=' + $pdf)
  }

  Write-Output 'STATUS=DONE'
} catch {
  Write-Output ('STATUS=ERR_COM: ' + $_.Exception.Message)
} finally {
  # Close only the presentation this script opened, never the user's.
  if ($pres) {
    try { $pres.Close() } catch {}
    try { [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($pres) } catch {}
  }
  # Quit only an instance this script started. When the user already had
  # PowerPoint open we merely borrowed it, and quitting would close their work.
  if ($app) {
    if ($null -ne $origAlerts) { try { $app.DisplayAlerts = $origAlerts } catch {} }
    if (-not $preExisting) {
      try { $app.Quit() } catch {}
    } else {
      Write-Output 'LEFT_USER_POWERPOINT_RUNNING=1'
    }
    try { [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($app) } catch {}
  }
  try { [GC]::Collect(); [GC]::WaitForPendingFinalizers() } catch {}
}

Write-Output ('WSL_WORKDIR=' + (ConvertTo-WslPath $workDir))
