
            void close() { } // in script, OK

            class MyClass implements Closeable {
                void close() {}
            }

            class MyClass2 {
                private void close() { } // private, OK
                void close(arg1) { } // with arguments, OK
            }

            class MyClass3 {
                def close() { null } // return type, OK
            }

            class MyClass4 implements AutoCloseable {
                void close() {}
            }
        