
            class MyClass {
                private static final String TEST_FILES = 'src/**/*Test.groovy'
                private static final String REGEX1 =  /\s*\*\//
                private static final String REGEX2 =  /.*\/*/

                void doStuff() {
                    String includes = '**/*.groovy'
                    String excludes = "temp/*File2*"
                }
            }
        