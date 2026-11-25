
             class MyTestCase extends TestCase {
                void testMethod() {
                    assert false == foo()
                    assert foo() == false : "message"
                }
              }
        