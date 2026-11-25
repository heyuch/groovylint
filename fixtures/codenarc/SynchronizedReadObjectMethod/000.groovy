
          // OK, class not Serializable
          class MyClass1 {
              private synchronized void readObject(ObjectInputStream input) throws IOException, ClassNotFoundException { }
          }

          // OK, class not Serializable
          class MyClass2 {
              private void readObject(ObjectInputStream input) throws IOException, ClassNotFoundException {
                  synchronized(lock) { }
              }
          }

          // OK, Synchronized block is sufficiently advanced
          class MyClass3 {
              private void readObject(ObjectInputStream input) throws IOException, ClassNotFoundException {
                  synchronized(lock) { }
                  doSomething()
              }
          }
        