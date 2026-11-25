
            class MyClass {
                private static final LOG = LoggerFactory.getLogger(this)
                def log2 = LoggerFactory.getLogger(this.class)
                private static log3 = LoggerFactory.getLogger(this.getName())
                private static log4 = LoggerFactory.getLogger(this.name)
            }
        