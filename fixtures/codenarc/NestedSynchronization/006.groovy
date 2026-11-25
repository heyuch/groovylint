
            class NestedSynchronizationClass7 {

                static class NestedSynchronizationClass8 {
                    def closure1 = {
                        synchronized(this) {
                            synchronized(this) {}
                        }
                    }
                    def myMethod() {
                        synchronized(this) {
                            synchronized(this) {}
                        }
                    }
                }
            } 