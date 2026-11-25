
            class MyClass {
                private static final LOG = LogFactory.getLog(this)
                def log2 = LogFactory.getLog(this.class)
                private static log3 = LogFactory.getLog(this.getName())
                private static log4 = LogFactory.getLog(this.name)
            }
        