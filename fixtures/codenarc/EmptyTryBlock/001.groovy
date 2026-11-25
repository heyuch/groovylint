
            class MyClass {
                void myMethod1()  {
                    try(FileReader reader = new FileReader("file.txt")) {
                    } catch(MyException e) {
                        e.printStackTrace()
                    }
                }
                
                void myMethod2()  {
                    try (def input = url.openStream(); def output = new ByteArrayOutputStream(4096)) {
                         println output
                    }
                    catch (IOException e) {
                        // some code to handle the exception
                    }
                }
                
                void myMethod3()  {
                    try (def input = url.openStream()) {
                         println input
                    }
                    catch (IOException e) {
                        // some code to handle the exception
                    }
                }
            }
        