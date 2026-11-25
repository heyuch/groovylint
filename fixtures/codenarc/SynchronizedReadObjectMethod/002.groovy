
            class MyClass implements Serializable {

                private void readObject(ObjectInputStream input) throws IOException, ClassNotFoundException {
                    synchronized(lock) {
                        // violation, no need to synchronized
                    }
                }
            }
      