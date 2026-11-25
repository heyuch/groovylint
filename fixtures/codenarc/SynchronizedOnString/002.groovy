
            class MyClass {

                final String stringLock = "stringLock"

                def method() {
                    synchronized(stringLock) { }
                }
            }
        