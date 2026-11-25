
            class NestedSynchronizationClass6 {

                def runnable = new Runnable() {
                    public void run() {
                        synchronized(this) {
                            synchronized(this) {}
                        }
                    }
                }
            } 