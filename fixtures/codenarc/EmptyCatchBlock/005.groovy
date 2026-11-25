
            class MyClass {
                void myMethod()  {
                    try (def input = url.openStream(); def output = new ByteArrayOutputStream(4096)) {
                         // Do stuff
                    }
                    catch (IOException e) {
                        println e
                    }
                }
                
                void myMethod2()  {
                    try (def input = url.openStream()) {
                         // Do stuff
                    }
                    catch (IOException e) {
                        println e
                    }
                }
            }
        