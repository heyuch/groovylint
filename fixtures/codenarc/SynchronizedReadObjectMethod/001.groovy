
            class MyClass implements Serializable {

                private synchronized void readObject(ObjectInputStream input) throws IOException, ClassNotFoundException {
                    // violation, no need to synchronized
                }
            }
      