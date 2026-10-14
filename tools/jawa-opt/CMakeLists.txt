add_llvm_executable(jawa-opt jawa-opt.cpp)
llvm_update_compile_flags(jawa-opt)
target_link_libraries(jawa-opt PRIVATE
  JawaDialect
  JawaTransforms
  MLIRControlFlowDialect
  MLIRFuncDialect
  MLIRFuncInlinerExtension
  MLIRLLVMDialect
  MLIROptLib
  MLIRTransforms
)
