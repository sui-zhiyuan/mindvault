# pptx-read / render_pptx.ps1
#
# Render a .pptx with the REAL PowerPoint engine (COM) into per-slide PNG,
# export a PDF, and dump every slide's text to UTF-8 text.md.
#
# READ-ONLY GUARANTEE: the source deck is copied into the scratch dir first and
# opened ReadOnly. The original file is never written to.
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

$root    = Join-Path $env:TEMP 'pptx-read'
$workDir = Join-Path $root $Slug
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

# Drop artifacts of any earlier run so a stale slide can never be mistaken for
# this deck's output.
Get-ChildItem -Recurse $workDir -Include *.PNG, *.pdf -ErrorAction SilentlyContinue |
  Remove-Item -Force -ErrorAction SilentlyContinue

$local = Join-Path $workDir 'deck.pptx'
Copy-Item -LiteralPath $src -Destination $local -Force
Write-Output 'STATUS=COPIED'

$app = New-Object -ComObject PowerPoint.Application
try { $app.DisplayAlerts = 1 } catch {}
$pres = $null
try {
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

  $pres.Close()
  Write-Output 'STATUS=DONE'
} catch {
  Write-Output ('STATUS=ERR_COM: ' + $_.Exception.Message)
} finally {
  try { $app.Quit() } catch {}
}

Write-Output ('WSL_WORKDIR=' + (ConvertTo-WslPath $workDir))
