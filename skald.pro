QT += core gui qml quick quickcontrols2 dbus network webenginequick

CONFIG += c++17 release
TARGET = skald
TEMPLATE = app

HEADERS += \
    src/backend.h \
    src/library.h \
    src/mpris.h \
    src/theme.h \
    src/translator.h

SOURCES += \
    src/main.cpp \
    src/backend.cpp \
    src/library.cpp \
    src/mpris.cpp \
    src/theme.cpp \
    src/translator.cpp

RESOURCES += src/resources.qrc
