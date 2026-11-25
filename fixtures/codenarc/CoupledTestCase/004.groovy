
            package com.example

            class MyTest extends GroovyTestCase {
                void testMethod() {
                    def input = MyTest.getResourceAsStream('sample.txt')
                    def otherInput = com.example.MyTest.getResourceAsStream('other.txt')
                }
            }
        