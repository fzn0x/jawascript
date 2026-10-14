add_mlir_dialect_library(JawaDialect
  JawaDialect.cpp
  JawaOps.cpp
  JawaMemorySlot.cpp

  ADDITIONAL_HEADER_DIRS
  ${PROJECT_SOURCE_DIR}/include/jawa/Dialect/Jawa/IR

  DEPENDS
  JawaOpsIncGen

  LINK_LIBS PUBLIC
  MLIRFuncDialect
  MLIRIR
  MLIRMemorySlotInterfaces
  MLIRSideEffectInterfaces
  MLIRTransformUtils
)
