
            class MyClass {
                private static final LOG = LogFactory.getLog(MyClass)
                def log2 = LogFactory.getLog(MyClass.class)
                private static log3 = LogFactory.getLog(MyClass.getClass().getName())
                private static log4 = LogFactory.getLog(MyClass.getClass().name)
                private static log5 = LogFactory.getLog(MyClass.class.getName())
                private static log6 = LogFactory.getLog(MyClass.class.name)
            }
        