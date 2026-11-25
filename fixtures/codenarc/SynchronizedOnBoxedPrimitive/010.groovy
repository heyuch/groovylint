
                class MyClass5 {
                    final Boolean lock = someMethod() // not interned, but quite possibly is
                    def method() {
                        synchronized(lock) { }
                    }
                }
            