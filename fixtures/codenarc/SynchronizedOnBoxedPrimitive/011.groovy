
                class MyClass {

                    final boolean lock = true

                    def method() {
                        synchronized(lock) { }
                    }
                }
            