
            class MyClass {
                private static final LOG = LoggerFactory.getLogger(MyClass)
                def log2 = LoggerFactory.getLogger(MyClass.class)
                private static log3 = LoggerFactory.getLogger(MyClass.getClass().getName())
                private static log4 = LoggerFactory.getLogger(MyClass.getClass().name)
                private static log5 = LoggerFactory.getLogger(MyClass.class.getName())
                private static log6 = LoggerFactory.getLogger(MyClass.class.name)
            }
        