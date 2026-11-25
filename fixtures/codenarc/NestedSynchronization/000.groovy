
            class testApplyTo_NoViolations_InClosureClass {
                def myMethod() {
                    synchronized(this) {
                        def closure1 = {
                            synchronized(this) { }
                        }
                        def closure2 = {
                            synchronized(this) { }
                        }
                    }
                }
            }
            