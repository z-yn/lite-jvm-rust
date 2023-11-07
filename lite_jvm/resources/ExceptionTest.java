public class ExceptionTest {
    private String name;

    public int throwNullPointException() {
        return name.length();
    }

    public void methodDeclareThrows() throws NullPointerException {
        throwNullPointException();
    }

    public int methodRecovery() {
        int i = 0;
        try {
            throwNullPointException();
            return -1;
        } catch (Exception e) {
            i = 2;
            return i;
        } finally {
            i = 3;
        }
    }

    public StackTraceElement[] methodStackTrace() {
        try {
            throwNullPointException();
        } catch (Exception e) {
            return e.getStackTrace();
        }
        return null;
    }
}
