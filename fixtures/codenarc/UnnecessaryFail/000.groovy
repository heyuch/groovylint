
            class MyTest {
                public void testMethod() {
                    try {
                        something()
                    } catch (Exception e) {
                        LOG.error(e) // OK
                        if (x) {
                            fail(e.message)
                        }
                    }

                    try {
                        something()
                    } catch (Exception e) {
                        fail('xyz', e)  //I suppose this is OK b/c it is not the JUnit Assert.fail()
                    }

                    try {
                        something()
                    } catch (Exception e) {
                        assert 'xyz' == e.message
                    }
                }
            }
        