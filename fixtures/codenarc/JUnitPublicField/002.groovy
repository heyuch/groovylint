
            class MyTestCase {
                @Rule public TestName testName = new TestName()

                @org.junit.Rule
                public TestName otherTestName = new TestName()
            }
        