
          class MyTest extends TestCase {
            void setUp() {
                println 'bad'
                super.setUp('But', 'has', 'parameters')
            }
          }
        