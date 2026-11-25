
                class MyClass5 {
                    final boolean lock = someMethod() // not interned, but quite possibly is
                    def method() {
                        synchronized(lock) { }
                    }
                }
            