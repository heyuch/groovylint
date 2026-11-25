
            class MyClass {
                private static field1
                protected static String field2 = 'abc'
                public static int field3 = 123
                static String property1 = 'abc'

                private void doStuff() {
                    println field1 + 'suffix'
                    def isReady = field2 > 'xxx'
                    println field3 == 999
                    property1 < 'xxx'
                }
            }
        