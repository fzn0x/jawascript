configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}/lit.site.cfg.py.in
  ${CMAKE_CURRENT_BINARY_DIR}/lit.site.cfg.py
  @ONLY)

set(JAWA_TEST_DEPENDS jawa-opt jawac jawa_rt)

# lit comes from `pip install lit`; scripts/test.sh sets up a virtualenv for it
# in the build tree.
find_program(JAWA_LIT NAMES lit llvm-lit
  HINTS
  ${PROJECT_BINARY_DIR}/.venv/bin
  ${PROJECT_BINARY_DIR}/.venv/Scripts
  ${LLVM_TOOLS_BINARY_DIR})

if(JAWA_LIT)
  add_custom_target(check-jawa
    COMMAND ${JAWA_LIT} -sv ${CMAKE_CURRENT_BINARY_DIR}
    DEPENDS ${JAWA_TEST_DEPENDS}
    COMMENT "Running the Jawa lit tests"
    USES_TERMINAL)
else()
  add_custom_target(check-jawa
    COMMAND ${CMAKE_COMMAND} -E echo "lit was not found; run `pip install lit` and reconfigure"
    COMMAND ${CMAKE_COMMAND} -E false)
endif()
