
            class MyClass {
                static final VALUE = 'value'
                private static final int OTHER = 99
                final String something = 'something'  // this is cleaner
                final String somethingElse = VALUE      // this is cleaner
                final String someClass = String      // this is cleaner

                @Override
                String getSomething(def parameter) {
                    //not a java getter
                }

                @Override
                String getSomethingElse() {
                    performSomeWork() // not a simple one liner
                    'something else'
                }
            }
        