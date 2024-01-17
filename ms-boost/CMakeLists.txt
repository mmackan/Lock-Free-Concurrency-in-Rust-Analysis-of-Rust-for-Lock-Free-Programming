cmake_minimum_required(VERSION 3.17)

option(USE_CLANG "build application with clang" ON)

set(CMAKE_C_COMPILER clang)
set(CMAKE_CXX_COMPILER clang++)

project(test_boost)

add_subdirectory(boost)

add_executable(queue queue.cpp)
target_link_libraries(queue Boost::thread Boost::atomic)