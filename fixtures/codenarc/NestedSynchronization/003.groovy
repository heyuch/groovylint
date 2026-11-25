
            class NestedSynchronizationClass4 {

                // within a method
                def myMethod() {
                    synchronized(this) {
                        synchronized(this) {}
                    }
                }
            } 