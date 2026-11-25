
            class NestedSynchronizationClass9 {
                def myMethod() {
                    synchronized(this) {}
                }
            }

            // synchronized within a second class
            class NestedSynchronizationClass2 {
                def myMethod() {
                    synchronized(this) {
                        synchronized(this) {}
                    }
                }
            }
            