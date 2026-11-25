
            class MyClass {

                final String stringLock = "stringLock"

                class MyInnerClass {
                    def method() {
                        // violation
                        synchronized(stringLock) { }
                    }
                }
            }
        