public class ThreadTest {
    public static void main(String[] args) {
        new Thread(()->Utils.localPrintln("New Thread Start")).start();
    }
}
