add_mlir_library(JawaPipeline
  Pipeline.cpp

  ADDITIONAL_HEADER_DIRS
  ${PROJECT_SOURCE_DIR}/include/jawa/Pipeline

  LINK_COMPONENTS
  Core
  MC
  Passes
  Support
  Target
  TargetParser
  native

  LINK_LIBS PUBLIC
  JawaDialect
  JawaToLLVM
  JawaTransforms
  MLIRArithDialect
  MLIRBuiltinToLLVMIRTranslation
  MLIRLLVMToLLVMIRTranslation
  MLIRPass
  MLIRTargetLLVMIRExport
  MLIRTransforms
)
