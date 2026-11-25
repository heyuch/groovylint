
            class MyClass {
                private static final LOG = LoggerFactory.getLogger(MyClass)
                private static final LOG2 = '' // not a logger
            }
            class MyOtherClass {
                private static final LOG3 = LoggerFactory.getLogger(MyClass)
                class MyInnerClass {
                    private static final LOG4 = LoggerFactory.getLogger(MyClass)
                }
            }
        