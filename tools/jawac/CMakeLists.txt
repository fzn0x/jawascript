add_llvm_executable(jawac main.cpp)
llvm_update_compile_flags(jawac)
target_link_libraries(jawac PRIVATE JawaBridge)

install(TARGETS jawac RUNTIME DESTINATION bin)
