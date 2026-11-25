
            class testApplyTo_NoViolations_InAnonymousInnerClass {
                def myMethod() {
                    synchronized(this) {
                        def runnable = new Runnable() {
                            public void run() {
                                synchronized(this) {}
                            }
                        }
                    }
                }
            }
            