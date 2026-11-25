
          class MyTest extends TestCase {
            void tearDown() {
                println 'bad'
                super.tearDown('But', 'has', 'parameters')
            }
          }
        