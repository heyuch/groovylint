
            class MyClass {
                private static final String TEST_FILES = 'src/**/*Test.groovy'
                
                void doStuff() {
                    String includes = '**/*.groovy'
                    String excludes = "temp/*File2*"
                }
            }
        