# PyCharm Workspace 子模块报 `No module named` 的 Bug 及修复

## 环境

- **IDE**: PyCharm Professional 2026.2 (Windows + WSL2)
- **项目结构**: uv workspace，根模块 `mindvault`，子模块 `daft-demo`
- **Python 解释器**: WSL 中的 venv（`~/projects/mindvault/.venv/bin/python`）
- **依赖安装**: `uv pip install ray`，venv 的 site-packages 中 `ray` 可正常导入

## 症状

在子模块 `daft-demo` 的 `.py` 文件中 `import ray`，PyCharm 编辑器报：

```
No module named 'ray'
```

但终端中 `python -c "import ray"` 正常运行，根模块文件也能正常解析。

## 根因

PyCharm 从 `pyproject.toml` 自动生成 workspace 子模块的 `.iml` 时，子模块的 content root 仅限项目源码目录：

```xml
<content url="file://$MODULE_DIR$/daft_demo">
  <sourceFolder url="file://$MODULE_DIR$/daft_demo/daft_demo" isTestSource="false" />
</content>
```

site-packages 在 content root 范围之外，PyCharm 无法解析外部 `import`。模块间不共享 content root。

## 修复

在子模块 `.iml` 中为 site-packages 添加第二个 content root：

```xml
<!-- 修改前 -->
<content url="file://$MODULE_DIR$/daft_demo">
  <sourceFolder url="file://$MODULE_DIR$/daft_demo/daft_demo" isTestSource="false" />
</content>

<!-- 修改后 -->
<content url="file://$MODULE_DIR$/daft_demo">
  <sourceFolder url="file://$MODULE_DIR$/daft_demo/daft_demo" isTestSource="false" />
</content>
<content url="file://$MODULE_DIR$/.venv/lib/python3.11/site-packages" />
```

或者通过 UI：`File → Settings → Project → Project Structure` → 选中子模块 → `+ Add Content Root` → 选择 `.venv/lib/python3.11/site-packages`。
