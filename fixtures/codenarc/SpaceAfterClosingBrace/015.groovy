
            def service = new MyService() { };
            def service2 = new MyService() {
                @Override
                void run() throws Exception {
                     println 123
                }
            };
        