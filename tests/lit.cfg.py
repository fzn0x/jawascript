# The lit configuration for the Jawa test suite. CMake writes the paths into
# lit.site.cfg.py in the build tree, which loads this file.

import os

import lit.formats
from lit.llvm import llvm_config

config.name = "Jawa"
# lit's internal shell, on every platform: LLVM 23 deprecates the external one.
config.test_format = lit.formats.ShTest(execute_external=False)
config.suffixes = [".mlir", ".jw", ".test"]
config.excludes = ["Inputs", "CMakeLists.txt", "README.md", "lit.cfg.py", "lit.site.cfg.py"]

config.test_source_root = os.path.dirname(__file__)
config.test_exec_root = os.path.join(config.jawa_obj_root, "tests")

llvm_config.with_system_environment(["HOME", "TMP", "TEMP", "SYSTEMROOT"])
llvm_config.use_default_substitutions()

# The tools and, on Windows, the LLVM and MLIR DLLs they load.
llvm_config.with_environment("PATH", config.llvm_tools_dir, append_path=True)
llvm_config.with_environment("PATH", config.jawa_tools_dir, append_path=True)

tool_dirs = [config.jawa_tools_dir, config.llvm_tools_dir]
llvm_config.add_tool_substitutions(["jawa-opt", "jawac"], tool_dirs)

config.substitutions.append(("%examples", os.path.join(config.jawa_src_root, "examples")))
