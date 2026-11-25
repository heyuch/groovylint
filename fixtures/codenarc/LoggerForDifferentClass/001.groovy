
            class MyClass {
                private static final LOG = LoggerFactory.getLogger(SomeOtherClass)
                def log1 = LoggerFactory.getLogger(SomeOtherClass.class)
                def log2 = LoggerFactory.getLogger(SomeOtherClass.class.name)
            }
        