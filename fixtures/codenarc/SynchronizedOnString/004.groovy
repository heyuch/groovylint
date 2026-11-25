
            class MyClass {
                // implicit typing
                final def stringLock = "stringLock"

                def method() {
                    // violation
                    synchronized(stringLock) { }
                }
            }
        