QT += core gui quick quickcontrols2 testlib dbus
CONFIG += c++17 testcase
TARGET = skald_tests
TEMPLATE = app

INCLUDEPATH += ../src

HEADERS += ../src/backend.h ../src/library.h ../src/theme.h ../src/translator.h
SOURCES += skald_tests.cpp ../src/backend.cpp ../src/library.cpp ../src/theme.cpp ../src/translator.cpp
RESOURCES += ../src/resources.qrc
