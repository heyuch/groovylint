
            class MyClass5 {
                final String lock = someMethod() // not interned, but quite possibly is
                def method() {
                    synchronized(lock) { }
                }
            }
        