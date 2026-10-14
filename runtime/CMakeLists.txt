# The runtime that compiled Jawa programs link against, built on the
# Boehm-Demers-Weiser garbage collector (bdwgc).

find_path(BDWGC_INCLUDE_DIR gc/gc.h
  DOC "Directory holding gc/gc.h from bdwgc")
if(WIN32)
  # Link the collector statically on Windows, so programs run without
  # libgc-1.dll next to them.
  find_library(BDWGC_LIBRARY NAMES libgc.a gc
    DOC "The bdwgc library")
else()
  find_library(BDWGC_LIBRARY NAMES gc
    DOC "The bdwgc library")
endif()
if(NOT BDWGC_INCLUDE_DIR OR NOT BDWGC_LIBRARY)
  message(FATAL_ERROR "bdwgc was not found. Install it with "
    "`apt install libgc-dev`, `brew install bdw-gc` or "
    "`pacman -S mingw-w64-ucrt-x86_64-gc`.")
endif()
message(STATUS "Using bdwgc: ${BDWGC_LIBRARY}")

add_library(jawa_rt STATIC jawa_rt.c)
target_include_directories(jawa_rt PRIVATE ${BDWGC_INCLUDE_DIR})
set_target_properties(jawa_rt PROPERTIES
  C_STANDARD 11
  C_STANDARD_REQUIRED ON)

# How jawac links a program: the object file, this runtime, the collector.
if(WIN32)
  set(link_args -static ${BDWGC_LIBRARY})
else()
  set(link_args ${BDWGC_LIBRARY} -lm)
endif()
set(JAWA_LINK_ARGS "${link_args}" CACHE INTERNAL "Extra arguments for linking Jawa programs")
