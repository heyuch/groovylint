
            class MyClass1 {
                protected final LOG = LoggerFactory.getLogger(this.class)
            }

            class MyClass2 {
                protected final LOG = LoggerFactory.getLogger(this.getClass())
            }

            class MyClass3 {
                protected final LOG = LoggerFactory.getLogger(getClass())
            }
        