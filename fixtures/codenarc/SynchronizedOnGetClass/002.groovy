
            class C {
                def m = {
                    synchronized(getClass()) {
                        // ...
                    }
                }
            } 