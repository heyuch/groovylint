
            class NestedSynchronizationClass5 {

                def closure1 = {
                    synchronized(this) {
                        synchronized(this) { }
                    }
                }
            } 