
            fileSystem.getFileSystem() //just a variable reference, not a FileSystem reference
            fileSystem.fileSystem.delete(aFile)

            new File() // no arg ctor allowed, b/c they don't exist in JDK
            new File(one, two, three) // three arg ctor allowed, b/c they don't exist in JDK

            new FileReader()  // no arg ctor allowed, b/c they don't exist in JDK
            new FileReader(one, two, three) // three arg ctor allowed, b/c they don't exist in JDK

            new FileOutputStream()  // no arg ctor allowed, b/c they don't exist in JDK
            new FileOutputStream(one, two, three)              new FileOutputStream(one, two, three)

            new RandomAccessFile(parm)
            new RandomAccessFile(one, two, three)
        