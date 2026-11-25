
            class MyTest {
                public void testMethod() {
                    try {
                        something()
                    } catch (Exception e) {
                        LOG.error(e)
                        fail()
                    }
                }
            }
        