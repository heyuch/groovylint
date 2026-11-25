
            enum MyEnum {
                ONE, TWO, THREE
                Object writeReplace(String name) { }
                private Object writeReplace(int count) { }
                private void writeObject(String name) throws IOException { }
            }
        