
            class MyClass {
                private static final long serialVersionUID = 1234567L
                private static final ObjectStreamField[] serialPersistentFields = { new ObjectStreamField("name", String.class) }
                String name;
                Object writeReplace() { }
                private void writeObject(ObjectOutputStream stream) throws IOException { }
            }
        