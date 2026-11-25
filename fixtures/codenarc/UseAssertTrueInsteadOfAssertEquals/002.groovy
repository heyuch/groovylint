
             class MyTestCase extends TestCase {
                void testMethod() {
                    assert true == foo()
                    assert foo() == true : "message"
                }
              }
        