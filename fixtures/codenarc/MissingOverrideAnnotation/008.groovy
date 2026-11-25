
            class OuterClass {
                Runnable runnable

                OuterClass() {
                    runnable = new Runnable() {
                        void run() {
                        }
                    }
                }
            }
        