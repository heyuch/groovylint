
            class MyTest extends GroovyTestCase {
                public void testMethod() {
                    // violation, static method call to other test
                    MyOtherTest.helperMethod()
                }
            }
        