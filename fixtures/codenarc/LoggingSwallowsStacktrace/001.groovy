
            class MyClass {
                private static final Log LOG = LogFactory.getLog(Main.class)
                private static final Log logger = LogFactory.getLog(Main.class)

                def method() {
                    try {
                        LOG.error(foo)
                    } catch (Exception e) {
                        if (true) {
                            LOG.error('error') // ok... surrounded by if
                        }
                        LOG.error(e) // violation
                        logger.error(e) // violation
                        foo(e)
                    }
                }
            }
        