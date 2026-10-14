add_mlir_conversion_library(JawaToLLVM
  JawaToLLVM.cpp

  ADDITIONAL_HEADER_DIRS
  ${PROJECT_SOURCE_DIR}/include/jawa/Conversion

  DEPENDS
  JawaConversionPassIncGen

  LINK_LIBS PUBLIC
  JawaDialect
  MLIRArithToLLVM
  MLIRControlFlowToLLVM
  MLIRFuncToLLVM
  MLIRLLVMCommonConversion
  MLIRLLVMDialect
  MLIRPass
  MLIRTransforms
)
