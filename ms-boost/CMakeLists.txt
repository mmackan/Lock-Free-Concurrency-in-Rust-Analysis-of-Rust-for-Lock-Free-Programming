cmake_minimum_required(VERSION 3.17)
project(test_boost)

add_subdirectory(boost)

add_executable(queue queue.cpp)
target_link_libraries(queue Boost::thread Boost::atomic)