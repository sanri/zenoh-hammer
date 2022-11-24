cmake_minimum_required(VERSION 3.5)

project(zenoh-hammer VERSION 0.1 LANGUAGES CXX)

set(CMAKE_INCLUDE_CURRENT_DIR ON)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(QT NAMES Qt6 REQUIRED COMPONENTS Widgets LinguistTools)
find_package(Qt${QT_VERSION_MAJOR} REQUIRED COMPONENTS Widgets LinguistTools)

set(TS_FILES src/ts/zenoh-hammer_zh_CN.ts)

set(PROJECT_SOURCES
        ${TS_FILES}
        src/main.cpp
        src/page_mainwindow/mainwindow.cpp
        src/page_mainwindow/mainwindow.h
        src/page_mainwindow/mainwindow.ui
        src/page_session/page_session.cpp
        src/page_session/page_session.h
        src/page_session/page_session.ui
        src/page_sub/page_sub.cpp
        src/page_sub/page_sub.h
        src/page_sub/page_sub.ui
        src/page_put/page_put.cpp
        src/page_put/page_put.h
        src/page_put/page_put.ui
        src/page_pub/page_pub.cpp
        src/page_pub/page_pub.h
        src/page_pub/page_pub.ui
        src/page_get/page_get.cpp
        src/page_get/page_get.h
        src/page_get/page_get.ui
        src/qzenoh/qzenoh.cpp
        src/qzenoh/qzenoh.h
        )

qt_add_executable(zenoh-hammer
        MANUAL_FINALIZATION
        ${PROJECT_SOURCES}
        )

qt_add_translations(zenoh-hammer TS_FILES
        src/ts/zenoh-hammer_zh_CN.ts
        src/ts/zenoh-hammer_en_US.ts
        )

if(NOT DEFINED ENV{ZENOHC_PATH})
    message(FATAL_ERROR "not defined environment ZENOHC_PATH")
endif()

set(ZENOHC_INCLUDE_PATH $ENV{ZENOHC_PATH}/include)
if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(ZENOHC_LIB_PATH $ENV{ZENOHC_PATH}/target/debug)
else()
    set(ZENOHC_LIB_PATH $ENV{ZENOHC_PATH}/target/release)
endif()

target_include_directories(zenoh-hammer PUBLIC ${ZENOHC_INCLUDE_PATH})
target_link_directories(zenoh-hammer PUBLIC ${ZENOHC_LIB_PATH})

target_link_libraries(zenoh-hammer PRIVATE
        Qt${QT_VERSION_MAJOR}::Widgets
        zenohc
        )

set_target_properties(zenoh-hammer PROPERTIES
    MACOSX_BUNDLE_GUI_IDENTIFIER my.example.com
    MACOSX_BUNDLE_BUNDLE_VERSION ${PROJECT_VERSION}
    MACOSX_BUNDLE_SHORT_VERSION_STRING ${PROJECT_VERSION_MAJOR}.${PROJECT_VERSION_MINOR}
    MACOSX_BUNDLE TRUE
    WIN32_EXECUTABLE TRUE
)

install(TARGETS zenoh-hammer
    BUNDLE DESTINATION .
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})

qt_finalize_executable(zenoh-hammer)
