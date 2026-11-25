
            class NestedSynchronizationClass1 {
                def myMethod() {
                    synchronized(this) {}
                }
            }

            // synchronized within a second class
            class NestedSynchronizationClass2 {
                def myMethod() {
                    synchronized(this) {}
                }
                def method2() {
                    synchronized(this) {}
                }
                def method3() {
                    synchronized(this) {
                        // synchronized block within a closure
                        def closure1 = {
                            synchronized(this) { }
                        }
                        def closure2 = {
                            synchronized(this) { }
                        }
                    }
                }
                // synchronized within a static inner class
                static class NestedSynchronizationClass3 {
                    def myMethod() {
                        synchronized(this) {}
                    }
                    def method2() {
                        synchronized(this) {}
                    }
                    def method3() {
                        synchronized(this) {
                            // synchronized block within a closure
                            def closure1 = {
                                synchronized(this) { }
                            }
                            def closure2 = {
                                synchronized(this) { }
                            }
                        }
                    }
                }
            }
            