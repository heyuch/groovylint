
                class MyClass {
                    // implicit typing
                    final def lock = true

                    def method() {
                        // violation
                        synchronized(lock) { }
                    }
                }
            