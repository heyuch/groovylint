
          class MyTest extends TestCase {
            void setUp() {
                println 'bad'
                other.setUp()
            }
          }
        