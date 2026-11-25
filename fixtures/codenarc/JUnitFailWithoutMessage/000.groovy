
             class MyTestCase extends TestCase {
    @Test
                void testMethod() {
                    fail('msg')
                    Assert.fail('msg')
                    this.fail('msg')
                    foo.fail()
                }
             }
        