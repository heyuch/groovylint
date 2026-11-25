
            class MyClass {
                private final def lock = new Object()

                def Method1() {
                    synchronized(this) { ; }
                }
                def Method2() {
                    synchronized(lock) { ; }
                }
                def Method3() {
                    synchronized(this) { ; }
                }
                def Method4() {
                    synchronized(lock) { ; }
                }
            }
        