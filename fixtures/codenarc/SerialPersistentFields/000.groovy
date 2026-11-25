
            class MyClass implements Serializable {
                private static final ObjectStreamField[] serialPersistentFields = [ new ObjectStreamField("myField", List.class) ] as ObjectStreamField[]
            }

            // not serializable
            class MyClass2 {
                ObjectStreamField[] serialPersistentFields = [ new ObjectStreamField("myField", List.class) ] as ObjectStreamField[]
            }

            // wrong field name
            class MyClass3 implements Serializable {
                ObjectStreamField[] zz_serialPersistentFields = [ new ObjectStreamField("myField", List.class) ] as ObjectStreamField[]
            }

        